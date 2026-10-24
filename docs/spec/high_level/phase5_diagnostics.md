# High Level Fase 5 Diagnostics

Diagnostics baseicos de Fase 5:

- `E_HIGH_DYN_TRAIT_UNKNOWN_001`: `dyn` referencia un trait inexistente.
- `E_HIGH_DYN_NON_TRAIT_001`: `dyn` referencia un tipo que no es trait.
- `E_HIGH_DYN_OBJECT_SAFETY_001`: trait object no es seguro para objeto.
- `E_HIGH_DYN_GENERIC_METHOD_001`: metodo generico no permitido en trait object Fase 5.
- `E_HIGH_DYN_METHOD_MISSING_001`: `dyn.call` referencia metodo inexistente.
- `E_HIGH_DYN_METHOD_SIGNATURE_001`: firma de llamada dinamica no coincide.
- `E_HIGH_DYN_UNSIZED_PLACEMENT_001`: trait object sin tamano fijo en almacenamiento directo.
- `E_HIGH_DYN_IMPLICIT_001`: trait usado como valor sin `dyn`.
- `E_HIGH_ASSOC_TYPE_DUP_001`: associated type duplicado.
- `E_HIGH_ASSOC_TYPE_MISSING_001`: impl sin binding requerido.
- `E_HIGH_ASSOC_TYPE_UNKNOWN_001`: binding no declarado por el trait.
- `E_HIGH_ASSOC_TYPE_AMBIGUOUS_001`: proyeccion no resoluble de forma unica.
- `E_HIGH_ASSOC_TYPE_BINDING_001`: binding incompatible con firma esperada.
- `E_HIGH_ANY_LEAK_001`: `any` intenta atravesar superficie dinamica.
- `E_HIGH_GENERIC_HKT_DEFERRED_001`: HKT sigue deferred.
- `E_HIGH_SPECIALIZATION_DEFERRED_001`: specialization sigue deferred.
