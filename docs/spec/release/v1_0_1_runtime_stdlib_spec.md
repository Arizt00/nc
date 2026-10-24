# nC Runtime + Stdlib Spec v1.0.1

Estado: `frozen`  
Serie: `1.x`  
Fase: `phase3_runtime_stdlib`

## Objetivo

Cerrar runtime de producción y stdlib base de extremo a extremo para los modelos:

- `hosted`
- `portable`
- `freestanding`

## Distribución como librería externa

Las familias de import runtime se distribuyen como componente externo del
toolchain bajo el paquete `ncc-runtime-stdlib`. El parser/sema mantienen la
superficie tipada del lenguaje, MCT mantiene los bytes de activación de cada
import, y el ejecutor/backend hosted resuelve las llamadas contra ese paquete.

Esto evita que un import parezca soportado sin backing real: cada entrada debe
seguir declarando `closed_real_backing_bytes`, `closed_native_backend`,
`closed_interpreter_runtime` o `catalog_only_with_diagnostic`.

Los proveedores dependientes del sistema operativo deben conservar semántica
estable. Ejemplo: `renderBinary` puede usar `snprintf("%llb")` en la libc del
host solo si una prueba en runtime confirma soporte real; si no, usa el emisor
portable exacto de nC.

Con superficie tipada, estable y verificable por contrato.

## Módulos base congelados

- `io`
- `mem`
- `collections`
- `net`
- `time`
- `trace`
- `inspect`

## Base de 90 familias runtime

Además del set base de módulos de fase 3, el base activo congela 90 familias runtime (`nC.runtime.*`) como superficie de importación/descubrimiento tipada.

- Fuente canónica: `docs/spec/root/base_inputs/nc_runtime_api_90_familias.md`
- Contrato machine-readable: `runtime_families` en `docs/spec/release/v1_0_1_runtime_stdlib_contract.json`
- CLI autoritativo: `ncc stdlib-model --list-runtime-families`

## API estable por módulo

### `io`

- `system.io.core.write(str) -> bool`

### `mem`

- `memory.heap.manager.alloc(int) -> addr`
- `memory.heap.manager.free(addr) -> bool`
- `memory.heap.manager.realloc(addr, int) -> addr`
- `memory.heap.manager.zero(addr, int) -> bool`

### `collections`

- `list<T>.push/get/set/len`
- `queue<T>.enqueue/dequeue/front/isEmpty`
- `watchList<T>.push/len`
- `traceList<T>.push/len`
- `memList<T>.push/get/set/len`

### `net`

- `system.net.core.connect(str, int) -> bool`
- `system.net.core.listen() -> bool`
- `system.net.core.accept() -> bool`
- `system.net.core.send(str) -> bool`
- `system.net.core.recv() -> bool`
- `system.net.core.disconnect() -> bool`
- `alt.net.core` mantiene espejo canónico compatible.

### `time`

- `runtime.time.now() -> int`
- `runtime.time.monotonic() -> int`
- `runtime.time.sleep(int) -> bool`

### `trace`

- `runtime.trace.mark(str) -> bool`
- `runtime.alias.mark(str) -> bool`

### `inspect`

- runtime paths canónicos: `heap.*`, `net.*`, `system.*`, `kernel.*`, `security.*`, `ai.*`
- salida estructurada por `print.inspect`, `print.mem`, `print.st`

## Alias canónicos

- `use stoud.memory { heap };` -> `memory.heap.manager`
- `use stoud.net { net };` -> `system.net.core` / `alt.net.core`
- `use stoud.audit { trace };` -> `runtime.trace`
- `use nC.runtime.alias;` -> `runtime.alias`
- `use stoud.time { time };` -> `runtime.time`
- `use nC.runtime.*;` -> wildcard canónico runtime (incluye `time`).

## Cierre tipado end-to-end

- parser/sema/runtime comparten la misma firma por import.
- no se permite cambio breaking de firmas en `1.x` sin RFC.
- los builtins de runtime deben mantener retorno y aridad estables.

## Validación obligatoria

Gate oficial de fase:

- `release_phase3_runtime_stdlib_gate_tests`

Cobertura mínima del gate:

1. artefactos de fase presentes y no vacíos;
2. contrato JSON congelado y consistente;
3. manifest runtime/stdlib en CLI consistente con contrato;
4. integración e2e ejecutable de `io`, `mem`, `collections`, `net`, `time`, `trace`, `inspect`.

## DoD

Fase 3 se considera cerrada solo si:

1. API stdlib base está congelada en `v1.0.1`;
2. runtime `hosted/portable/freestanding` mantiene contrato estable;
3. gate `release_phase3_runtime_stdlib_gate_tests` está en verde.
