# High Level Object Safety

Object safety define si un trait puede convertirse en trait object.

Reglas Fase 5:

- El trait debe existir y ser trait.
- Los metodos llamados por `dyn.call` deben tener firma resoluble.
- Metodos genericos por defecto no son object-safe para Fase 5.
- Associated types deben estar resueltos por el impl concreto antes de construir el objeto.
- No se acepta `any` como escape de tipos.

Diagnostics:

- `E_HIGH_DYN_OBJECT_SAFETY_001`
- `E_HIGH_DYN_GENERIC_METHOD_001`
- `E_HIGH_ASSOC_TYPE_MISSING_001`
- `E_HIGH_ASSOC_TYPE_BINDING_001`
- `E_HIGH_ANY_LEAK_001`
