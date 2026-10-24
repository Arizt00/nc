# nC RFC Process (1.x)

Este directorio contiene la base de gobernanza de cambios de nC para serie `1.x`.

## Regla principal

No se permiten cambios breaking en `1.x` sin RFC aprobado.

## Flujo mínimo

1. crear RFC desde `RFC_TEMPLATE.md`;
2. completar impacto de compatibilidad y clasificación semver;
3. enlazar cambios de parser/sema/IR/runtime/backend;
4. actualizar gates/documentación;
5. aprobar RFC antes de integrar.

## Convención sugerida

- archivo: `RFC-YYYYMMDD-<slug>.md`
- carpeta opcional de aprobadas: `approved/`

