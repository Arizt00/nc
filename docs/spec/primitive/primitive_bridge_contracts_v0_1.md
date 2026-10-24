# nC v0.1 — `[@primitive]` bridge contracts

## Estado
Documento canónico operativo para la frontera `[@primitive]` ↔ `[@]` / `[@machine]` en la línea actual.

## Representación tipada
`bridge:` en `[@primitive]` ya no se representa como texto libre.

AST/Sema usan `PrimitiveBridgeTarget`:
- `RootPath(Vec<String>)`
- `MachinePath(Vec<String>)`

IR usa `PrimitiveBridgeTargetIR`:
- `RootPath(Vec<String>)`
- `MachinePath(Vec<String>)`

## Superficie válida de `bridge:`
| Forma | Estado |
|---|---|
| `bridge: root.policy.bind;` | Permitido |
| `bridge: root.policy.guard;` | Permitido |
| `bridge: root.memory.resolve;` | Permitido |
| `bridge: root.binary.attach;` | Permitido |
| `bridge: machine.<container>;` | Permitido |
| `bridge: machine.<container>.<label>;` | Rechazado |
| `bridge: rootToPrimitive;` | Rechazado |
| `bridge: system.net.connect;` | Rechazado |

## Reglas semánticas activas
- Si target es `root.<...>`, se valida contra contratos root conocidos.
- Si target es `machine.<container>`, el contenedor machine debe existir en el programa.
- Targets machine deben usar forma exacta `machine.<container>`.
- Se mantienen reglas estructurales previas de primitive:
  - máximo una sección `bridge:` por contenedor.
  - `bridge:` requiere al menos una sección `anchor:`.
  - `seal:` requiere `bridge:`.
  - `seal:` debe ser la última sección.
- Continuidad profunda por subfamilias (sema):
  - `timing`, `signal`, `flow` y `fault` requieren bridge `machine.<container>`.
  - `signal` requiere bridge `root.policy.bind`.
  - `flow/fault` requieren bridge `root.policy.guard`.
  - `memory` requiere bridge `root.memory.resolve`.

## Diagnósticos canónicos
| Condición | Diagnóstico |
|---|---|
| target no cualificado o no permitido en parser | `bridge section in [@primitive] only allows 'root.<...>' or 'machine.<...>', got '...'` |
| target machine no canónico | `machine bridge target '...' must use official form 'machine.<container>'` |
| target machine inexistente | `machine bridge target 'machine.<name>' not found` |
| target root inexistente | `root symbol 'root....' not found` |
| símbolo root físico no enlazado | `root physical symbol 'root....' not linked` |

## Notas de continuidad
- `[@primitive]` declara frontera material; no ejecuta ISA.
- `[@machine]` conserva ejecución tipada y control de flujo propio.
- `bridge:` es el punto formal de continuidad entre capas sin degradación stringly.
- Matriz de diagnósticos detallada: `primitive_diagnostics_official_v0_1.md`.
