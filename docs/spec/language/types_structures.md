# Types and Structures

## Cobertura
Tipos base, genéricos, ptr/region, arrays/tensores, Result/Option, structs/enums, uniones.

Incluye `type:` como DSL contractual formal:

- secciones internas de contrato (`TypeContractSection`):
  `public/private/internal/strictly`, `layout`, `bounds`, `invariant`,
  `constructure`, `destructure`, `copy`, `clone`, `move`, `ffi`,
  `ownership`, `nullability`, `methods`, `associated`, `validate`, `drop`,
  `default`, `compare`, `hash`, `debug`, `stable`.
- no son secciones globales del contenedor.
- no requieren `[@primitive]`.
- se validan por parser/sema con códigos `E_TYPE_*` estables.

## Regla
Cada tipo define operadores y casts válidos por modo.

## Related docs
- `docs/spec/language/type_contracts.md`
- `docs/spec/language/bounds_type_contracts.md`
- `docs/spec/compiler/compiler_type_contract_sections.md`
- `docs/spec/runtime/runtime_type_contracts.md`
