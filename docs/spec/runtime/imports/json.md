# Runtime JSON

`nC.runtime.json` expone un DOM JSON tipado mediante handles `int` opacos y
poseídos. El handle `0` es inválido; los hijos devueltos por navegación son
copias independientes y se liberan con `jsonRelease`.

```n
use <data:json>;
```

El import explícito permanece soportado, pero no es obligatorio en código
high-level alojado ni en `[@runtime]`: `nC.runtime.json` forma parte del
`hosted runtime prelude v1`. Esta disponibilidad es semántica y de backend;
no aparece como `UseDecl` sintético en `Program.uses`. Root `[@]`,
`[@machine]` y `primitive[@]` no reciben el preludio.

La familia contiene 46 funciones para:

- parsear, validar, cargar, guardar y serializar;
- consultar y convertir valores tipados;
- recorrer objetos, arrays y JSON Pointer RFC 6901;
- construir, clonar, mutar, eliminar y liberar árboles;
- consultar códigos, mensajes, ubicación y ruta del último error.

Los límites por defecto son 16 MiB de entrada, 128 niveles, 1 000 000 de nodos
y 65 536 handles vivos. NaN, infinito, UTF-8 inválido y sustitutos Unicode
incompletos se rechazan.

En ejecución alojada, la familia usa `ncc-runtime`. El compilador añade el
objeto nativo con las 46 implementaciones únicamente cuando LLVM referencia
una función `json*`; Machine Phase 12 exige cero símbolos sin resolver. El
proveedor se compila como COFF/PE, ELF o Mach-O según el target.

Contrato detallado: [`runtime_json.md`](../runtime_json.md).

Evidencia ejecutable:

- `examples/runtime/families/json_basic.n`;
- `examples/runtime/json/json_complete_ok.n`;
- `examples/runtime/json/json_native_pe_status_ok.n`;
- `crates/ncc-cli/tests/runtime_json_native_pe.rs`.

El preludio no cambia el contrato de vida v1: los documentos y nodos continúan
siendo handles `int`, y el programa debe llamar `jsonRelease`. No representa
ownership automático, arena concurrente ni scheduler async.
