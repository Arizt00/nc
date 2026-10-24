# nC v0.1 — `[@primitive]` — diagnósticos canónicos (parser + sema)

## Estado
Matriz canónica activa para `primitive[@]`, alineada con parser/sema de la línea actual.

## Regla de autoridad
1. Parser valida forma/superficie (`bridge`, secciones, directivas tokenizadas).
2. Sema valida contratos estructurales y continuidad `primitive ↔ machine ↔ root`.
3. No existe fallback stringly para `bridge` ni para directivas tipadas.

## Diagnósticos canónicos de parser

| Condición | Diagnóstico canónico |
|---|---|
| Sección no permitida dentro de `[@primitive]` | `section not allowed in [@primitive] context` |
| Sección primitive usada fuera de `[@primitive]` | `section not allowed outside [@primitive] context` |
| Target `bridge` no canónico/no permitido | `bridge section in [@primitive] only allows 'root.<...>' or 'machine.<...>', got '...'` |
| Token inválido en directiva tipada | `unexpected token in primitive directive section: ...` |

## Diagnósticos canónicos de sema

| Condición | Diagnóstico canónico |
|---|---|
| Sección vacía | `empty <section> section is not allowed in [@primitive] container` |
| Símbolo duplicado por familia | `duplicate <section> symbol '...' in [@primitive] container` |
| Target `bridge` duplicado | `duplicate bridge target '...' in [@primitive] container` |
| Más de una sección `bridge` | `multiple bridge sections are not allowed in the same [@primitive] container` |
| Más de una sección `seal` | `multiple seal sections are not allowed in the same [@primitive] container` |
| `bridge` sin `anchor` | `bridge section requires at least one anchor section in [@primitive] container` |
| `seal` sin `bridge` | `seal section requires at least one bridge section in [@primitive] container` |
| `seal` fuera del final | `seal section must be the last section in [@primitive] container` |
| Item de directiva con múltiples tokens | `invalid <kind> directive item '...': expected a single scalar token` |
| Item de directiva con puntuación | `invalid <kind> directive item '...': scalar token cannot be punctuation` |
| Cláusula de directiva sin payload escalar | `invalid <kind> directive item '...': expected scalar payload after directive head in [@primitive] container` |
| Cláusula `await` mal formada | `invalid await clause '...': expected 'await <expr> timeout <number> cycle[s] [else fault <name>]' in [@primitive] container` |
| Cláusula `settle` mal formada | `invalid settle clause '...': expected 'settle <number> cycle[s]' in [@primitive] container` |
| Cláusula `access` mal formada | `invalid access clause '...': expected 'access <symbol>' in [@primitive] container` |
| Cláusula `risk` mal formada | `invalid risk clause '...': expected 'risk <symbol>' in [@primitive] container` |
| Cláusula `stable` mal formada | `invalid stable clause '...': expected 'stable after <number> cycle[s]' in [@primitive] container` |
| Cláusula `high` mal formada | `invalid high clause '...': expected 'high means <symbol>' in [@primitive] container` |
| Cláusula `read side_effect` mal formada | `invalid read side_effect clause '...': expected 'read side_effect <symbol>' in [@primitive] container` |
| Cláusula `capability` mal formada | `invalid capability clause '...': expected 'capability <path>' in [@primitive] container` |
| Cláusula `requires` mal formada | `invalid requires clause '...': expected 'requires <expression>' in [@primitive] container` |
| Cláusula `source` mal formada | `invalid source clause '...': expected 'source <path>' in [@primitive] container` |
| Cláusula `field` mal formada | `invalid field clause '...': expected 'field <path>' in [@primitive] container` |
| Cláusula `window` mal formada | `invalid window clause '...': expected 'window <path>' in [@primitive] container` |
| Cláusula `cache` mal formada | `invalid cache clause '...': expected 'cache <symbol>' in [@primitive] container` |
| Cláusula `owner` mal formada | `invalid owner clause '...': expected 'owner <path>' in [@primitive] container` |
| Cláusula `immutable` mal formada | `invalid immutable clause '...': expected 'immutable after <symbol>' in [@primitive] container` |
| Referencia `operation -> capability` no resuelta | `operation '<op>' in [@primitive] references unknown capability '<path>' in clause '...'` |
| Referencia `operation -> port` (`reads/writes`) no resuelta | `operation '<op>' in [@primitive] references unknown port '<path>' in clause '...'` |
| Referencia `port -> bus` no resuelta | `port '<port>' in [@primitive] references unknown bus '<path>' in clause '...'` |
| Referencia `signal -> source(port)` no resuelta | `signal '<signal>' in [@primitive] references unknown port '<path>' in clause '...'` |
| Referencia `signal -> field(bitfield)` no resuelta | `signal '<signal>' in [@primitive] references unknown bitfield '<path>' in clause '...'` |
| Referencia `signal -> field(bitfield.entry)` no resuelta | `signal '<signal>' in [@primitive] references unknown bitfield field '<bitfield>.<entry>' in clause '...'` |
| Cardinalidad `operation.capability` inválida | `operation '<op>' in [@primitive] has multiple capability clauses` |
| Cardinalidad `window.requires signature` inválida | `window '<window>' in [@primitive] has multiple requires-signature clauses` |
| Cardinalidad `signal.source` inválida | `signal '<signal>' in [@primitive] has multiple source clauses` |
| Cardinalidad `signal.field` inválida | `signal '<signal>' in [@primitive] has multiple field clauses` |
| Correlación `signal.field -> source` faltante | `signal '<signal>' in [@primitive] requires source clause when field clause is declared` |
| Referencia `signature -> window` no resuelta | `signature '<sig>' in [@primitive] references unknown window '<path>' in clause '...'` |
| Referencia `window -> signature` (`requires signature`) no resuelta | `window '<window>' in [@primitive] references unknown signature '<path>' in clause '...'` |
| Referencia `entry -> anchor` no resuelta | `entry '<entry>' in [@primitive] references unknown anchor '<path>' in clause '...'` |
| Referencia `entry -> signature` no resuelta | `entry '<entry>' in [@primitive] references unknown signature '<path>' in clause '...'` |
| Escalar de directiva incompatible con su contrato | `invalid <kind> directive scalar '...': expected <class> scalar in [@primitive] container` |
| Alias `type` duplicado | `duplicate type alias symbol '...' in [@primitive] container` |
| Alias `type` sin valor | `empty type alias value for '...' is not allowed in [@primitive] container` |
| Base de `bitfield` no canónica | `unsupported bitfield base type '...' in bitfield '...' of [@primitive] container` |
| Índice `bit` no numérico | `bitfield entry '...' in bitfield '...' requires numeric bit index, got '...'` |
| Inicio/fin de rango `bits` no numérico | `bitfield entry '...' in bitfield '...' requires numeric range start/end, got '...'` |
| Rango `bits` invertido | `bitfield entry '...' in bitfield '...' has invalid range <start>..<end> (start must be <= end)` |
| Índice/rango fuera de base | `bitfield entry '...' in bitfield '...' uses out-of-range bit(s) ... for base '...' (width ...)` |
| Solape de bits entre entradas | `bitfield entry '...' in bitfield '...' overlaps bit <n> already assigned` |
| Target machine bridge no canónico | `machine bridge target '...' must use official form 'machine.<container>'` |
| Target machine bridge inexistente | `machine bridge target 'machine.<name>' not found` |
| Target root inexistente | `root symbol 'root....' not found` |
| Símbolo root físico no enlazado | `root physical symbol 'root....' not linked` |
| `timing` sin bridge machine | `timing directive family requires at least one bridge target 'machine.<container>' in [@primitive] container` |
| `signal` sin bridge machine | `signal directive family requires at least one bridge target 'machine.<container>' in [@primitive] container` |
| `flow` sin bridge machine | `flow directive family requires at least one bridge target 'machine.<container>' in [@primitive] container` |
| `fault` sin bridge machine | `fault directive family requires at least one bridge target 'machine.<container>' in [@primitive] container` |
| `memory` sin `root.memory.resolve` | `memory directive family requires bridge target 'root.memory.resolve' in [@primitive] container` |
| `flow/fault` sin `root.policy.guard` | `flow/fault directive families require bridge target 'root.policy.guard' in [@primitive] container` |
| `signal` sin `root.policy.bind` | `signal directive family requires bridge target 'root.policy.bind' in [@primitive] container` |
| Handshake `valid` sin `ready` | `signal handshake in [@primitive] requires 'ready' when 'valid' is declared` |
| Handshake `ready` sin `valid` | `signal handshake in [@primitive] requires 'valid' when 'ready' is declared` |
| Handshake `ack` sin `valid/ready` | `signal handshake in [@primitive] requires both 'valid' and 'ready' when 'ack' is declared` |

## Nota
Este documento define la matriz de diagnóstico de F3 para la superficie activa; cualquier expansión futura debe versionarse con su propia matriz canónica.
