# Runtime Standard Imports v0.1

Este documento cierra la superficie compacta del runtime estandar para codigo `.n`
normal. El objetivo es reducir dispersion sin esconder el estado real del backend.

## Familias baseicas

```nc
use stoud.collections { array, list, result };
use stoud.io { output, render, input, random, fsread };
use stoud.memory { ptr, heap, region };
```

Estas familias son el punto de entrada preferente para codigo normal. Los imports
legacy `use nC.runtime.*;` siguen aceptados por compatibilidad, pero la forma
baseica para ejemplos nuevos es `use <familia:items>`.

## Backing real

Un import queda marcado como `closed_real_backing` cuando tiene una de estas
rutas:

- lowering de lenguaje ya materializado, como `array`;
- ejecucion real en runtime, como `input`, `output`, `render`, `random`,
  `fsread`, `fswrite`, `ptr`, `heap`, `region`, `list` y `result`;
- bytes de activacion MCT/backend asociados al simbolo de importacion.

`fsread` y `fswrite` son alias de superficie hacia `nC.runtime.fs`. Las funciones
runtime visibles son `readFile(path)` y `writeFile(path, text)`.

## Catalogo sin backing

El catalogo amplio de `stoud/auth/dom` sigue existiendo para no romper la linea
futura del lenguaje. Cuando un item existe en catalogo pero no esta bajado al
runtime/backend real, el manifest debe marcarlo con:

```text
E_USE_BACKING_DEFERRED_001: existe en catalogo pero no esta bajado a backend
```

Esto evita dos errores: rechazar imports que ya forman parte del base futuro, o
presentarlos como si ya tuvieran ejecucion real.

## Comandos de comprobacion

```bash
ncc imports manifest --json
ncc imports backend examples/runtimeStdPunto9.n --json
ncc check examples/runtimeStdPunto9.n
ncc run examples/runtimeStdPunto9.n
```
