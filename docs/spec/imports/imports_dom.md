# dom Imports

`dom` conserva alias de compatibilidad para dominios especializados:

```n
use <dom.ai:model, agent, memory, policy, eval>;
use <dom.compiler:parse, ast, sema, lowering, diagnostics, report, manifest>;
use <dom.high:typeinfo, metadata, consteval, comptime, macros>;
use <dom.inspect:site, metadata, trace, report, evidence>;
use <dom.tooling:fmt, lint, lsp, package, build>;
```

Estos `use` no son necesarios para invocar las API manuales actuales. Los
simbolos `compiler*`, `ai*`, `high*`, `inspect*` y `tooling*` se resuelven desde
el prelude de backend. Todos reciben memoria del llamador y bajan a
LLVM/MCT/Machine; no crean objetos ni llaman a un proveedor C.

Cada familia usa registros raw de 64 bytes, salvo el workspace compiler de 128
bytes y sus tablas de tokens, nodos, IR y diagnosticos. Salir de capacidad,
pasar null o solapar buffers conserva semantica raw.
