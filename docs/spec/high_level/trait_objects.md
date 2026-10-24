# High Level Trait Objects

Un trait object representa un valor concreto detras de una interfaz de trait dinamica.

```n
let obj: dyn Drawable<Pixel> = dynCast<Drawable<Pixel>>(pixel);
```

El objeto conserva:

- trait nominal.
- argumentos genericos del trait.
- tipo concreto de origen cuando se construye con `dynCast`.
- layout semantico de vtable.
- mapa de associated types resueltos.

Los trait objects no pueden almacenarse como `stc` directo si el perfil no define representacion de tamano fijo. El compilador emite `E_HIGH_DYN_UNSIZED_PLACEMENT_001`.

No se permite tratar una interface o trait nominal como valor sin `dyn`; eso emite `E_HIGH_DYN_IMPLICIT_001`.
