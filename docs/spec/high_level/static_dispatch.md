# High Level Static Dispatch

Fase 4 resuelve metodos de interfaces y traits en tiempo de compilacion.

Caso concreto:

```n
rank(4)
```

Si existe un unico impl compatible con `rank(int) -> int`, la llamada se acepta.

Caso generico:

```n
fn hasRank<T: Rank<T>>(value: T) -> bool {
    public:
        return rank(@value) > 0;
}
```

El compilador usa el bound activo para conocer la firma de `rank`. Al instanciar la funcion, exige un impl compatible para el tipo concreto.

Reglas:

- Un solo candidato valido.
- No se aceptan referencias locales unresolved.
- Los impls ambiguos fallan.
- No hay trait objects ni vtables en esta fase.

Diagnostic principal: `E_HIGH_TRAIT_AMBIGUOUS_IMPL_001`.
