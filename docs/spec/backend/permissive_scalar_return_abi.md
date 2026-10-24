# ABI permisivo de retornos escalares

Fecha: 2026-07-21

## Contrato

El tipo declarado fija el ABI de la función, pero no obliga a que la expresión
fuente tenga exactamente ese tipo cuando ambos valores pertenecen a la clase
escalar material.

```n
discard() -> void {
    return 42;
}

flag() -> bool {
    return 7;
}

number() -> int {
    return true;
}
```

Reglas:

- `void`: evalúa la expresión, conserva sus efectos y descarta el valor;
- `bool`: normaliza cualquier escalar aceptado a `0` o `1`;
- enteros, `char`, `byte`, direcciones y punteros: convierten por el ancho y la
  operación ABI declarada;
- floats: usan conversión FP o entero-FP explícita en IR;
- agregados incompatibles no se reinterpretan como escalares;
- el registro, slot de stack o `sret` sigue determinado por el tipo declarado.

La libertad de retorno no elimina el ABI. Una función declarada `bool` nunca
expone bytes arbitrarios al llamador: su valor observable está normalizado.

## Backend

Sema registra la coerción admitida. LLVM emite `zext`, `trunc`, comparaciones con
cero o conversiones FP según corresponda. Native/MCT usa la misma clasificación
escalar y normaliza `bool` antes de `ret`. Para `void`, el valor no se asigna a un
registro de retorno.

Las pruebas `permissive_return_abi_tests` y
`permissive_return_abi_lowering_tests` cubren sema, LLVM y backend nativo.
