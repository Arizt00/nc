# nC Imports Operational Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`  
Fase: `imports_operational_closure`

## Declaración explícita

En nC v2.0.0, **todos los imports canónicos activos están en funcionamiento**.

Esto incluye:

1. Resolución de `use ...;` en parser/AST.
2. Validación semántica de visibilidad, autoridad modal y firma.
3. Ejecución runtime de llamadas importadas dentro del catálogo canónico.

Por contrato operativo, cuando un import se llama respetando su contrato (módulo válido, contexto válido, firma válida y precondiciones runtime de la operación), **no debe producir error de import/resolución**.

## Alcance cubierto

1. Catálogo runtime canónico de 90 familias:
   `docs/spec/root/base_inputs/nc_runtime_api_90_familias.md`
2. Catálogo generado:
   `crates/ncc-ast/src/import_catalog/runtime_api_catalog_generated.rs`
3. Alias y wildcard runtime (`nC.runtime.*`) en parser/sema/runtime.
4. Superficies operativas importadas en lanes activas:
   - `system.*`
   - `tooling.*`
   - `compiler.*`
   - `kernel.*`
   - `ai.*`
   - `security.*`
   - `memory.*`
   - `runtime.*`
   - `alt.net.*`

## Contrato de no-error al llamar imports

Una llamada importada DEBE ejecutarse sin error de import/resolución si:

1. El import pertenece al catálogo canónico activo.
2. Existe `use` visible (directo, wildcard o alias canónico).
3. El contexto modal tiene autoridad para ese import.
4. La función existe en el módulo importado.
5. La aridad/tipos satisfacen el contrato de la firma.

Nota: errores de negocio/precondición runtime (por ejemplo, `send` sin conexión activa) no contradicen este contrato; son validaciones funcionales del propio servicio runtime, no fallos de infraestructura de imports.

## Diagnósticos canónicos (cuando no se cumple contrato)

1. `unresolved import path '<path>'`
2. `function '<fn>' not found in import '<module.path>'`
3. `invalid context linking ... import:<module.path> ...`
4. `multiple profiled main entries found; select a profile explicitly` (si afecta entrypoint con imports en flujo de arranque)

## Gate de cierre

El cierre queda ligado al gate:

- `imports_operational_gate_tests` (verificado por script oficial)

y a los tests existentes de runtime/import en `ncc-cli` y catálogo en `ncc-ast`.
