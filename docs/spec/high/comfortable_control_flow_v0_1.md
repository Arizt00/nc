# nC High Control Flow Compacto v0.1

Este contrato cierra la superficie compacta de `guard`, `match`, `defer/final` y
`Result` permisivo para codigo `.n` normal.

## Guard compacto

Forma baseica comoda:

```n
guard p != null else return false;
```

Tambien se conserva la forma explicita:

```n
guard (p != null) else {
    return false;
}
```

`else` puede ser un bloque o una unica sentencia. La condicion acepta
comparacion de punteros con `null` mediante `==` y `!=`.

## Match compacto y Result permisivo

`match` ya no exige parentesis alrededor del scrutinee:

```n
match readInt() {
    ok(v) => numero = v;
    err(e) => dev(e @runtime);
}
```

Si el valor evaluado es `Result<T,E>`, se aplica el contrato cerrado normal. Si
el valor no es `Result`, `ok(v)` lo captura como valor crudo. Si el valor es un
diagnostico runtime, `err(e)` puede capturarlo. Esto es deliberadamente comodo y
peligroso: reduce verbosidad, pero no garantiza exhaustividad material.

## Defer y Final de funcion

`defer` y `final` son sentencias equivalentes dentro de `fn`:

```n
defer free(p);
final cleanup = true;
```

El cuerpo diferido se registra en una pila LIFO y se ejecuta al salir de la
funcion, incluso si la salida ocurre por `return`. El analizador valida el cuerpo,
pero no aplica sus efectos de puntero en el punto de registro.

Fixture oficial:

```bash
cargo run --bin ncc -- check examples/controlFlujoComodoPeligrosoOficial.n
cargo run --bin ncc -- run examples/controlFlujoComodoPeligrosoOficial.n
cargo run --bin ncc -- emit-ir examples/controlFlujoComodoPeligrosoOficial.n
```
