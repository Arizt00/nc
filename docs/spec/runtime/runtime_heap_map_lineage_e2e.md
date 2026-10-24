# Runtime heap, map y lineage

## Contrato de almacenamiento

La unidad canónica del heap es el byte. Una petición `alloc(n)` representa `n`
bytes, con independencia del tipo usado después para observar esa dirección. El
tipo solo determina stride, ancho y alineación de read/write.

La aritmética raw de `ptr<T>` puede salir de allocation o región. Esto no convierte
las vistas acotadas en raw: arrays, buffers, slices, vectors y regiones conservan su
propio contrato de límites. Acceder a una dirección no mapeada en ejecución nativa
puede producir el fallo real del sistema operativo.

## Realloc transaccional

1. Validar generación, procedencia y petición.
2. Reservar una dirección no solapada para el hijo.
3. Copiar `min(old_size, new_size)` bytes cuando corresponda.
4. Crear el hijo con `parent=old`, `root=old.root`, `generation=old+1`.
5. Marcar al padre como `reallocated` y enlazar su lista de hijos.
6. Publicar los eventos `realloc-out` y `realloc-in` con un sequence coherente.

Un fallo antes del paso 4 no mata ni modifica el bloque anterior.

## Mapeos del host

`map*` devuelve una dirección respaldada por memoria virtual real. `mapProtect`,
`mapSync`, `mapAdvise`, `mapLock`, `mapUnlock`, `remap*` y `unmap` operan sobre el
mapping del host. Read/write de la abstracción mapping validan su longitud y
protección; el programador puede obtener `mapAddress` y continuar por la vía raw.

El remap crea una nueva generación material cuando cambia la dirección. El objeto
anterior conserva su dirección histórica y apunta al hijo; nunca se reescribe el
pasado para simular continuidad.

## Eventos observables

Cada bloque conserva address, size, live/death, origin, creator/binder, parent,
root, generation, lineage path, children, siblings, event id, alloc/realloc/free
sequence, last mutation/actor y trace. El límite del trace se expresa con
`trace_truncated` y `trace_dropped_count`; no se oculta pérdida de historial.

## Diagnósticos

Se mantienen diagnósticos para null, origen ausente, stale generation, type/alignment
mismatch, use-after-free, double-free, overflow de dirección, mapping inválido,
protección incompatible y operación fuera del mapping. La ausencia deliberada de
allocation bounds en raw pointer no elimina esos contratos.
