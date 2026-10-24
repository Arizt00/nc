# Compiler List Imports (v2.1.0)

Implemented:
- runtime list alias module mapping: `runtime.list` -> `runtime.queue` contracts.
- wildcard list alias imports.
- individual list symbol imports (`use nC.runtime.list.listCreate;`).
- generic export-name normalization for symbol matching (`listCreate<T>` callable as `listCreate`).

Commands:
- `ncc runtime exports nC.runtime.list`
- `ncc runtime exports nC.list`
