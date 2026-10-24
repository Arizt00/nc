# nC v0.1 — [@machine] — superficie oficial mínima

## Estado
Documento operativo de implementación inicial dentro del repo.

## Propósito
Fijar la superficie visible mínima de `[@machine]` como dialecto ejecutable legible de bajo nivel.

## Secciones oficiales mínimas de [@machine]
Un contenedor `[@machine]` admite en esta fase únicamente:

- `reg:`
- `flag:`
- `label:`
- `inst:`
- `final:`

## Principios de cierre mínimo
1. `[@machine]` no es `[@]`.
2. `[@machine]` no es `primitive[@]`.
3. `[@machine]` es la capa ejecutable legible mínima del núcleo triádico.
4. Esta fase no cierra todavía toda la ISA; solo la superficie mínima auditable.

## Semántica mínima por sección

### `reg:`
Zona de declaración simbólica de registros mínimos del contenedor machine.

### `flag:`
Zona de declaración simbólica de flags mínimos del contenedor machine.

### `label:`
Zona de etiquetas de salto y localización simbólica.

### `inst:`
Zona de instrucciones machine mínimas legibles.

### `final:`
Clausura terminal del contenedor machine.
