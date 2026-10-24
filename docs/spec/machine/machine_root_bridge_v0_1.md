# nC v0.1 — `[@machine]` ↔ `[@]` — puente canónico

## Estado
Documento operativo del bloque D para fijar frontera machine↔root sin deriva semántica.

## Representación tipada
`call` en machine usa `MachineCallTarget`:

- `Label(String)`
- `RootPath(Vec<String>)`

No existe fallback stringly para mezclar rutas root con labels.

## Superficie válida de `call` en `[@machine]`
| Forma | Estado |
|---|---|
| `call -> <label_local>` | Permitido |
| `call -> root.policy.bind` | Permitido |
| `call -> root.policy.guard` | Permitido |
| `call -> root.memory.resolve` | Permitido |
| `call -> root.binary.attach` | Permitido |
| `call -> root.memory.map` | Rechazado |
| `call -> root.binary.resolve` | Rechazado |
| `call -> system.net.connect` | Rechazado |

## Frontera semántica
- machine no puede abrir linking libre hacia cualquier ruta cualificada.
- machine solo cruza a root por contratos implícitos baseizados.
- símbolos root físicos quedan fuera de autoridad machine en esta fase.
- `jmp`/`jmp.*`/`final:jmp` permanecen en labels locales.

## Diagnósticos clave
| Condición | Diagnóstico |
|---|---|
| símbolo root físico en call machine | `root physical symbol '...' not allowed in [@machine] context` |
| símbolo root inexistente | `root symbol '...' not found` |
| ruta cualificada no-root en call machine | `invalid machine instruction in [@machine] inst section: '...'` |
