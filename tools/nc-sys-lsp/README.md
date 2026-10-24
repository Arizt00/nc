# nc-sys-lsp (incremental funcional)

Estado: incremental funcional con edicion low-level estrictamente manual.

Propósito:

- cubrir asistencia low-level para `primitive[@]`, `[@machine]` y `[@]`;
- complementar `nclsp` sin duplicar autoridad semántica de `ncc`;
- habilitar flujos editoriales expertos con diagnóstico incremental por documento.
- completar por extensión y contexto en `.nm`, `.nb`, `.nbb` y `.ncp`;
- resolver arquitectura, registros, instrucciones y símbolos locales en Machine;
- resolver sites, bindings, routines, capabilities, anchors y operations declarados.

Este componente NO define semántica de lenguaje.

Autoridad semántica: `ncc`.

## Superficie CLI actual

- `nc-sys-lsp info`
- `nc-sys-lsp doctor`
- `nc-sys-lsp analyze <file> [--profile novice|standard|expert] [--migration-stage warning-first|error-by-profile|strict] [--json]`
- `nc-sys-lsp serve`
- `nc-sys-lsp lsp` (JSON-RPC 2.0 stdio adapter compatible con ciclo LSP)

## `serve` incremental (line-delimited JSON)

Entrada por línea JSON:

```json
{"id":"1","op":"open","path":"kernel.nb","text":"...","profile":"expert","migration_stage":"error-by-profile"}
{"id":"2","op":"diagnose","path":"kernel.nb"}
{"id":"3","op":"change","path":"kernel.nb","text":"..."}
{"id":"4","op":"close","path":"kernel.nb"}
{"id":"5","op":"capabilities"}
```

Salidas JSON por línea con `status`, `report` y diagnósticos lane-aware.
El `report` incluye `lane`, `editorial_profile_min` y `suggested_path` para migración controlada. Las familias canónicas son `.nb` basal, `.nm` Machine, `.nbb` Primitive y `.ncp` hardware físico.
Cada diagnóstico puede incluir `line`, `column`, `end_line` y `end_column` en
coordenadas humanas 1-based. El adapter LSP convierte esos campos a rangos
LSP 0-based para que VS Code/JetBrains puedan saltar directamente al marcador
real del problema.

## `lsp` adapter (JSON-RPC / Content-Length)

`nc-sys-lsp lsp` implementa ciclo base:

- `initialize`
- `initialized`
- `textDocument/didOpen`
- `textDocument/didChange`
- `textDocument/didClose`
- `textDocument/diagnostic`
- `textDocument/completion`
- `shutdown`
- `exit`

El adapter publica `textDocument/publishDiagnostics` y mantiene la misma regla doctrinal:
autoridad semántica en `ncc`; `nc-sys-lsp` complementa la lane low-level.

`initialize` no anuncia `completionProvider`. Si un cliente envia de todos
modos `textDocument/completion`, `.nm`, `.nb`, `.nbb` y `.ncp` responden con
cero elementos. No hay snippets, prediccion, simbolos sugeridos ni fallback
generico. Esas superficies conservan diagnosticos, colores, navegacion y
referencias, pero la escritura queda completamente en manos del programador.

Los diagnósticos LSP incluyen:

- `range` real;
- `code` estable (`NC-SYS-LSP-*`);
- `source = ncc`;
- ABI, fase, dominio, `stable_id`, archivo, span, ayuda y causas canónicas;
- `data` con los campos canónicos y el contexto editorial `family`, `lane`,
  `required_extension` y `suggested_path`.

Las publicaciones consumen `nc.diagnostic.machine.v2`. La persistencia común
usa JSONL o `.ndiag`; el servidor no define un registro semántico privado ni
incluye rutas o enlaces dependientes del checkout de desarrollo.

## Semantic token legend (contractual)

Types:
- `mode`, `section`, `type`, `function`, `method`, `variable`, `parameter`, `property`, `operator`, `number`, `string`, `comment`
- `runtimeFamily`, `compilerPhase`, `primitiveEntity`, `machineRegister`, `machineInstruction`
- `ffiSymbol`, `capability`, `policy`, `diagnostic`, `copyClone`, `destructure`, `constructure`

Modifiers:
- `readonly`, `unsafe`, `strict`, `primitiveOnly`, `machineOnly`, `runtimeOnly`, `deprecated`
- `experimental`, `error`, `warning`, `evidence`, `sealed`, `redacted`

## License

Licensed under either of:

- MIT license
- Apache License, Version 2.0

at your option.

SPDX-License-Identifier: MIT OR Apache-2.0
