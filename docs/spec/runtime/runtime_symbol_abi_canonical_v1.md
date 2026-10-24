# Runtime Symbol ABI Canonical v1

Estado: `closed`

## Identidad de enlace

Toda funcion runtime invocable tiene una identidad de enlace canonica bajo
`nC.runtime.*`. Los alias de superficie siguen aceptados, pero nunca aparecen como
identidades distintas en objetos o manifiestos.

- `nC.math.*` se normaliza a `nC.runtime.math.*`.
- `nC.vector.*` se normaliza a `nC.runtime.vector.*`.
- Los argumentos genericos pertenecen al tipo, no al nombre enlazable:
  `nC.runtime.result.ok<T>` enlaza como `nC.runtime.result.ok`.
- La comparacion del catalogo ignora mayusculas/minusculas de aliases; la salida
  siempre conserva la escritura canonica.

La autoridad de esta regla es `canonical_runtime_symbol` en el catalogo AST.

## Forma de llamada

Cada fila de `ncc runtime abi --json` declara:

- simbolo y aliases;
- parametros con nombre, tipo, ownership y paso;
- retorno y layout;
- payloads de `Result<T,E>`;
- errores, ciclo de vida y visibilidad;
- convencion logica `nC.runtime-value-v1`;
- adaptador fisico por destino: Win64, SysV64, AAPCS64, RISC-V psABI o
  WASM canonical.

No existe ya el fallback `RuntimeValue[] -> RuntimeValue`: las firmas se derivan
del catalogo runtime tipado y las excepciones se declaran de forma explicita.

## Importacion y exportacion

El manifiesto ABI `nc.runtime.symbol_abi.v1` incluye dos vistas con el mismo
conjunto y las mismas firmas:

- `import_manifest`: consumidor, esquema `nc.runtime.link_manifest.v1`;
- `export_manifest`: proveedor, esquema `nc.runtime.link_manifest.v1`.

Ambas usan enlace `external`, visibilidad `default` y verifican que no haya
duplicados, genericos en nombres de linker ni simbolos fuera de `nC.runtime.*`.

## Frontera

Este contrato fija el ABI logico del lenguaje. No promete que `RuntimeValue` sea
un ABI C crudo: MCT/LLVM adapta cada tipo al ABI material del target. Machine y
basal cruzan efectos runtime mediante bridges tipados, no mediante imports de alto
nivel introducidos implicitamente.

## Evidencia

- `runtime_linker_symbols_are_canonical_and_generic_free`.
- `runtime_symbol_abi_manifest_has_stable_shape`.
- `runtime_symbol_abi_covers_all_runnable_import_symbols`.
- Suite `ncc-cli`: 1027 aprobadas, 0 fallos (2026-07-10).
