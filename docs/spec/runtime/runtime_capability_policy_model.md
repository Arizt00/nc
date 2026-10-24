# Runtime Capability/Policy Model

## Capability
Define *qué* puede hacer una operación.

## Policy
Define *bajo qué reglas* puede hacerse.

## Regla de evaluación
1. Resolver símbolo runtime.
2. Validar capability requerida.
3. Validar policy requerida.
4. Emitir diagnóstico estable si falta contrato.

## CLI
- `ncc runtime capabilities --json`
- `ncc runtime policies --json`
