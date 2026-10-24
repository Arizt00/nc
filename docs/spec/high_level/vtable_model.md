# High Level VTable Semantic Model

Fase 5 define un layout semantico de vtable para trait objects. No fija ABI binaria final ni lowering runtime definitivo; eso queda para fases posteriores.

Cada `VTableLayout` contiene:

- trait nominal.
- tipo concreto.
- slots de metodo ordenados semanticamente.
- firma baseica por slot.
- associated type bindings resueltos.
- estado de object safety.

`dyn.call(@obj, method, args...)` selecciona un slot por nombre de metodo y valida la firma antes de aceptar la llamada.

El modelo preserva static dispatch: las llamadas con bounds genericos siguen resolviendose estaticamente cuando no usan `dyn`.
