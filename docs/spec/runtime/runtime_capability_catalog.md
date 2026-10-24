# Runtime Capability Catalog

## Alcance
Catálogo no mínimo de capabilities runtime por dominios:
- input/output,
- text/parse/render,
- memory/region/ptr,
- fs/process/net,
- security/policy/sandbox,
- compiler bridge contracts.

## Inspección
`ncc runtime capabilities --json`

## Reglas
- ID canónico estable.
- Metadata suficiente para auditoría.
- Sin capability, operación denegada.
