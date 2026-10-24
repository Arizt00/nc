# Runtime

## Pointer-backed regions

`p<T>` is a raw address. `rd`, `wr`, `off`, `dist`, `adr`, `from`, `alloc`,
`realloc`, `free`, `map` and `protect` lower to the same typed runtime/backend
ABI without hidden ownership, bounds or lifetime work.

The public runtime documentation tracks supported import families:

- `use stoud.io { input, output, render, random };`
- `use stoud.memory { ptr, heap, region };`
- `use stoud.collections { array, list, result };`
- `use stoud.net { tcp };`
- `use auth.system { process, thread, time };`

`print` and `println` require `use stoud.io { output };`. String, terminal, time,
mobile, network/system backend services and the direct filesystem operations
declared as prelude symbols do not require source imports.

Each public import should either have real backing or a clear diagnostic.

## Input Text

`readLine()` returns an owned runtime `str`. In the LLVM/MCT path it allocates
a 1024-byte heap buffer, reads up to 1023 input bytes, consumes the rest of an
overlong line, and writes an explicit trailing NUL. This keeps the returned
pointer valid after the helper function returns, so it can be stored in
`array<str, N>` or persistent state. `readFloat()` consumes its line separator
on that path so a subsequent `readLine()` begins at the next user response.

## Rich Output Colors

`color(text, name)` and `colorBold(text, name)` accept ANSI names, Spanish
aliases, CSS-style color names, truecolor hex (`#RGB`, `#RRGGBB`,
`0xRRGGBB`), `rgb(r,g,b)` / `RGB(r,g,b)` and 256-color forms such as
`ansi256:202`.

Derived names are also accepted, for example `rojo neon`, `azul profundo`,
`verde claro`, `dark orange`, `lightblue` and `rebecca purple`. The runtime
emits real terminal escape bytes unless color output is disabled.
