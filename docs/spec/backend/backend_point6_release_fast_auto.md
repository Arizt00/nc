# Backend Point 6: release-fast automatico y medible

Status: closed for the normal `ncc build` surface.

## Cierre

`ncc build app.n` activa el perfil `release-fast` por defecto sin llamar a
`ncc optimize` ni a ningun comando especial.

Cuando el usuario no pasa `-O`/`--opt-level`, el build normal conserva:

- `profile: release-fast`;
- `optimization.requested: auto`;
- `optimization.effective: O3`;
- `speed_contract.release_fast_automatic: true`;
- `speed_contract.special_optimize_command_required: false`;
- `speed_contract.measurable_release_fast: true`;
- `release_fast_auto.schema: nc.compiler.release_fast_auto.v1`.

## Evidencia retenida

El manifiesto `*.native-build.json` incluye `release_fast_auto` con:

- ruta de backend usada (`native-fast-backend` o `standard-mct-machine`);
- perfil efectivo;
- tiempos medidos de build;
- bytes de `.optimizer.json`, `.backend-object.nbo`, `.obj`, `.entry.obj` y `.exe`;
- contadores de optimizador cuando la ruta usa el optimizador interno;
- contrato de verdad que declara que `ncc optimize` no es requisito para el build.

## Integracion

El cierre esta integrado en:

- ruta directa `native-fast-backend`;
- ruta `standard-mct-machine` usada por targets que pasan por LLVM/MCT;
- fallback LLVM/MCT, que hereda `--profile release-fast` y por tanto O3.

## No declarado

Esto no afirma que cualquier programa nC ya iguale a C en todos los casos. Afirma
que el pipeline normal ya ejecuta y mide el perfil release-fast automaticamente, y
que el manifiesto retiene la evidencia necesaria para comparar contra C en CI o
benchmarks.
