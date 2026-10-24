# Politica de materializacion destructiva v2

## Alcance

`nC.destructive` sigue siendo un perfil de compilador. No incorpora Bear, no
cambia la gramatica de nC y no transforma `[@machine]` ni `primitive[@]`.
Lexer, parser y sema conservan sus contratos ordinarios. La seleccion ambigua
ocurre en IR tipado y sus resultados llegan a LLVM, objetos, relocalizaciones y
bytes ejecutables reales.

La politica tipada `nc.backend.destructive_policy.v2` conserva:

- materializacion de sentencias como `once`, `omit`, `repeat` o `split`;
- espacio de direcciones unificado;
- aritmetica de punteros modular;
- vida de memoria exclusivamente manual;
- fallos entregados a la maquina;
- ABI seleccionado por el backend cuando falta una eleccion;
- simbolos externos todavia ausentes materializados como direccion `0`.

## Simbolos ausentes

Los contratos `foreign` validados por sema se conservan en IR con lenguaje,
ABI, linkage, parametros y retorno. MCT genera una declaracion LLVM tipada y
Clang produce la relocalizacion externa normal del formato objetivo.

Durante el enlace destructivo, todos los objetos se combinan primero. Una
definicion real siempre gana. Solo los simbolos pedidos por el objeto principal
que continuan sin definicion dejan de ser imports dinamicos y pasan al conjunto
`null_materialized_symbols`. El aplicador escribe el valor `0` en relocalizaciones
absolutas, llamadas directas y entradas GOT admitidas. No se emite warning.

El informe Phase12 conserva `missing_symbol_policy`,
`null_materialized_symbols`, `dynamic_imports` y cada relocalizacion aplicada.
Esto es evidencia del byte producido, no un diagnostico durante la compilacion.

Los perfiles normales no cambian: un simbolo externo reconocido por el objeto
permanece como import dinamico y un simbolo no clasificado sigue produciendo
`E_MACHINE_LINK_UNRESOLVED_SYMBOL_001`.

Si se pide `--linker` junto a `nC.destructive`, nCC puede elegir silenciosamente
el enlazador interno para garantizar la materializacion nula. El manifiesto
separa `configured_linker_requested`, `configured_linker` aplicado y
`configured_linker_ignored_by_destructive_policy`.

## WSL

Desde la raiz del repositorio:

```bash
cargo build -p ncc-cli --bin ncc

target/debug/ncc build \
  examples/10_mct/destructive_missing_symbol.n \
  target/destructive-missing-symbol \
  --out-dir target/destructive-missing-evidence \
  --profile release-fast \
  --cc nC.destructive \
  --target x86_64-unknown-linux-gnu \
  --no-incremental

jq '.link.phase12_report | {
  missing_symbol_policy,
  null_materialized_symbols,
  dynamic_imports,
  relocations_applied
}' target/destructive-missing-evidence/run/destructive_missing_symbol.native-build.json
```

El ejecutable del ejemplo ausente contiene llamadas a la direccion nula. Su
construccion es la prueba; ejecutarlo puede terminar mediante fallo de maquina.

## Ejecucion nativa

`ncc run --cc nC.destructive` conserva el perfil al entrar en el constructor
MCT, transporta el triple objetivo y ejecuta el binario materializado. Esta
ruta no usa el interprete ni el enlazador alojado rapido: LLVM genera el objeto
y el enlazador interno aplica la misma politica de simbolos nulos que `build`.

Las fixtures ejecutables son:

- `examples/10_mct/destructive_execute_console.n`, que vuelve observables las
  cuatro decisiones de materializacion mediante salida escrita en nC;
- `examples/10_mct/destructive_execute_pointer.n`, que ejecuta reserva,
  escritura, lectura y liberacion manual de memoria.

La seleccion explicita de `--cc`, `--target`, `--cpu` o `--tune-profile` hace
que un fallo nativo sea un fallo de `run`; no habilita retorno silencioso al
interprete.

## Evidencia minima

Las fixtures estan en `examples/10_mct`. Cada build retiene `.standard.mct.ll`,
`.backend-object.nbo`, `.nm`, objeto objetivo, objeto de entrada, mapa de enlace,
manifiesto y ejecutable de cargador del sistema. `interpreter_fallback` permanece
en `false`.
